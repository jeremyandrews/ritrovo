# Ritrovo: The Editorial Workflow

How a conference gets from the importer to the public site, who is allowed to
move it, and what to type to watch it happen.

This is tutorial material and a record of what was verified. Every numbered step
below was run against the demo stood up from empty volumes with
`scripts/serve-demo.sh` on 2026-09-18, and the responses quoted are the ones it
gave. Where the kernel cannot do something the brief asks for, the step says so
and names the `FRICTION.md` entry rather than pretending.

**Kernel:** Trovato `v0.102.0`, rev `20baa121810b5c656b3f80028335770069fab5e0`.

---

## The pipeline

```
confs.tech ──▶ Incoming ──▶ Curated ──▶ Live ──▶ the public site
   importer     editor       publisher
```

Three stages, configured in `demo/config/stage.*.yml`, plus a fourth,
`legal_review`, which exists to show that adding a stage is configuration rather
than code. Each is a `category_tag` in the synthetic `stages` category with a
`stage_config` row carrying its machine name and visibility.

| Stage | Visibility | Who can see what is on it |
| :- | :- | :- |
| Incoming | internal | editors, publishers, administrators |
| Curated | internal | editors, publishers, administrators |
| Legal Review | internal | editors, publishers, administrators |
| Live | public | everyone, including anonymous visitors |

**Visibility is enforced by the kernel, not by a template.** `ItemService::check_access`
denies an unauthenticated viewer any item on an internal stage before it asks any
plugin, and for a signed-in viewer it asks `ritrovo_access`, whose `Deny` is
final. A conference on Incoming answers **404** to a visitor: not a styled "no
access" page, but the absence a 404 is built from, which is the right answer
because the existence of an unreviewed conference is itself not public.

Only one stage may be public: `uq_stage_visibility_public` is a partial unique
index, so "promote to Live" is the only way to make anything public and there is
no second public stage to leak through.

---

## The three editorial accounts

Created by `ritrovo_access`'s `tap_install` when the demo bootstraps. **These
passwords are published because the point of the accounts is that you log in as
each one and see a different site.** They exist only on a demo database.

| User | Password | Role | What they can do that the one above cannot |
| :- | :- | :- | :- |
| `viewer_carol` | `ritrovo-viewer-demo` | viewer | read Live |
| `editor_alice` | `ritrovo-editor-demo` | editor | see Incoming and Curated, edit any conference, promote Incoming to Curated |
| `publisher_bob` | `ritrovo-publisher-demo` | publisher | publish Curated to Live, and unpublish |
| `admin` | `ritrovo-demo-password` | administrator | everything, plus `/admin` |

Log in at `/user/login`.

### Why a plugin creates them rather than the registration form

The brief's recipe is to register the three and activate them by SQL, and that is
what this did first. It cannot work on this kernel. **The registration rate limit
is counted twice per request** — once in the middleware and once in the handler —
so the configured three per hour admits **one** account per hour. On a completely
clean install the first POST succeeds, the second is refused by the handler and
the third by the middleware, each with a different error shape.
`FRICTION.md`, `G-RATE-LIMIT-COUNTED-TWICE`.

To watch registration itself, register a fourth account by hand at
`/user/register`. Exactly one will succeed per hour, for the reason above. It
will land inactive and with no role, and nothing in the kernel can activate it or
give it one: there is no admin screen for user-to-role assignment, and
`trovato user` has a single subcommand, `reset-password`.

---

## Walkthrough 1: follow a conference from import to the public site

1. **Stand the demo up from nothing.**

   ```sh
   docker compose -f docker-compose.demo.yml down -v
   scripts/serve-demo.sh
   ```

   Wait for the import queue to drain. It takes a few minutes; watch it with
   `docker compose -f docker-compose.demo.yml exec -T postgres psql -U trovato -d ritrovo -tAc 'select count(*) from plugin_queue'`.

2. **See where the importer put everything.**

   ```sh
   docker compose -f docker-compose.demo.yml exec -T postgres psql -U trovato -d ritrovo \
     -c "SELECT sc.machine_name, count(i.id) FROM stage_config sc
         LEFT JOIN item i ON i.stage_id = sc.tag_id AND i.type = 'conference'
         GROUP BY 1 ORDER BY 1;"
   ```

   On the verified run: `incoming 5561`, `curated 20`, `live 75`, `legal_review 0`.

   Everything the importer fetched is on **Incoming**. It gets there because the
   importer writes its own `INSERT` through the raw-SQL host call and binds
   `stage_id` itself. No other path can: `item-api`'s `save-item` hardcodes
   `stage_id: None`, and a configuration set cannot name a stage for an item at
   all (`G-CONFIG-ITEM-CANNOT-NAME-A-STAGE`), which is why the seeded conferences
   in `demo/config/seed-content` are on Live whatever anyone wants.

   The Curated and Live counts are the demo seed: `ritrovo_access`'s `tap_cron`
   publishes the soonest 60 upcoming conferences and curates the next 20, once,
   when enough of the import has arrived. It stands in for editorial work nobody
   is going to do by hand on five thousand rows, and it runs the same UPDATE the
   editorial screen runs.

3. **Look at it as a visitor.** Open `/conferences` in a private window. The
   listing is full, and every conference on it is on Live. Pick one and note its
   id from the URL.

4. **Try to reach an unreviewed one as a visitor.** Take an id from Incoming:

   ```sh
   docker compose -f docker-compose.demo.yml exec -T postgres psql -U trovato -d ritrovo -tAc \
     "select id from item where type='conference'
      and stage_id='0193a5a0-0000-7000-8000-000000000002' limit 1"
   ```

   Visiting `/item/<that id>` signed out gives **404**. Signed in as
   `viewer_carol`, still **404**: being signed in is not editorial standing.

5. **Log in as `editor_alice` and visit the same URL.** **200.** The same item,
   the same kernel, a different viewer. That difference is `tap_item_access`
   returning `Grant` where it returned `Deny`, and it is the kernel that asked.

6. **Open the editorial queue** at `/admin/content/editorial`. Three tabs,
   Incoming, Curated and Live, with a count each, a checkbox per conference, and
   the transitions you are allowed to make. As `editor_alice` from Incoming
   there is one button: **Promote to Curated**.

7. **Promote a few.** Tick two or three, press the button. The page comes back
   with `Moved 3 conference(s) from incoming to curated.` and the counts move.

8. **Log in as `publisher_bob` and open the Curated tab.** Two buttons now:
   **Publish to Live** and **Send back to Incoming**. `editor_alice` sees neither
   of those on Curated: publishing is the publisher's.

9. **Publish.** Tick the conferences you just curated, press **Publish to Live**.
   On the verified run: `Moved 2 conference(s) from curated to live.`

10. **Check as a visitor.** `/item/<id>` signed out is now **200**, and the
    conference appears in `/conferences`. Verified end to end on the run above.

### The one wrinkle in step 10

A promoted conference appears in every **listing** immediately, because listings
query the database, and its **own page** can lag, because the kernel caches items
in process, keyed by id, and no host call lets a plugin invalidate that cache
(`G-PLUGIN-CANNOT-INVALIDATE-THE-ITEM-CACHE`). On a default install that is up to
five minutes of 404 on a page that is already linked from a listing.

The demo sets `CACHE_TTL_ITEMS: 10` in `docker-compose.demo.yml` so the promotion
looks immediate. **That is a demo setting covering a kernel gap, not a
recommendation.** The editorial screen says so on the page after every move,
rather than letting an editor conclude the promotion failed.

---

## Walkthrough 2: who may do what, and how they were told

1. **Look at the roles as configuration.** `demo/config/role.*.yml`, five files:
   `anonymous user`, `authenticated user`, `viewer`, `editor`, `publisher`.
   Administrator is not among them because it is not a role: it is the `is_admin`
   flag on a user, and it bypasses every check in this document.

2. **Import them and watch the permissions land.**

   ```sh
   docker compose -f docker-compose.demo.yml exec -T postgres psql -U trovato -d ritrovo \
     -c "SELECT r.name, count(rp.permission) FROM roles r
         LEFT JOIN role_permissions rp ON rp.role_id = r.id GROUP BY 1 ORDER BY 1;"
   ```

3. **Notice what is missing.** The brief asks for seven permissions:
   `view incoming`, `view curated`, `edit conferences`, `publish conferences`,
   `post comments`, `edit own comments`, `edit any comments`. `ritrovo_access`
   declares all seven through `tap_perm`. **Not one of them is held by any role,
   and none can be.**

   | Brief's permission | Declared by | What the demo uses instead |
   | :- | :- | :- |
   | `view incoming conferences` | `ritrovo_access` | `edit any content` |
   | `view curated conferences` | `ritrovo_access` | `edit any content` |
   | `edit conferences` | `ritrovo_access` | `edit any content` |
   | `publish conferences` | `ritrovo_access` | `delete any content` |
   | `post comments` | `trovato_comments` | nothing; comments are Part 6 |
   | `edit own comments` | `trovato_comments` | nothing; comments are Part 6 |
   | `edit any comments` | `trovato_comments` | nothing; comments are Part 6 |

   **Why.** The kernel never dispatches `tap_perm`, so a plugin's permissions
   reach no registry, no permission grid and no config-import validation. Config
   import rejects a permission string it has never seen, and the grid at
   `/admin/people/permissions` cannot render one. `G-PERM-TAP-NOT-DISPATCHED`.

   **There is a loophole, and this demo deliberately does not use it.** Config
   import accepts a permission the kernel defines *or that some role in the
   database already holds*, and `trovato_comments` seeds its three onto its own
   role when it installs. Because that install happens before the config import
   in `scripts/demo-bootstrap.sh`, naming the comment permissions in the role
   files did import cleanly — and then failed everywhere else. The
   host-in-the-loop suites import the same set into a bare database where that
   plugin has never installed, and since import is all-or-nothing, three
   ungrantable strings failed all 94 files and wrote nothing.

   The lesson is worth more than the three permissions: **naming another plugin's
   permission in a configuration set makes the set depend on that plugin's
   install order.** `demo/config` now names only permissions the kernel itself
   defines, so it imports into any Trovato database.

4. **Understand the cost of the substitution.** `edit any content` is a site-wide
   kernel permission, so a Ritrovo editor is an editor of every item type rather
   than of conferences only. That is a real widening of access, recorded in
   `FRICTION.md` and in the role files themselves. `delete any content` is worse:
   the kernel has no permission that means "publish", because promoting a stage
   is not an operation it models, so nothing means the same thing and
   `delete any content` is used purely as the marker that separates a publisher
   from an editor.

   `ritrovo_access` reads the brief's permission **first** and the substitute
   second, so the day the kernel dispatches `tap_perm` the role files gain four
   lines and nothing else changes.

5. **Do not save the permission grid.** Saving `/admin/people/permissions`
   rebuilds every role's permissions from the checkboxes it rendered, and it
   renders kernel permissions only, so a save revokes every plugin permission
   from every role — including the three comment permissions the demo depends on.
   `G-PERM-GRID-SAVE-REVOKES-PLUGIN-GRANTS`.

---

## Walkthrough 3: bulk publish

**It works, and it is not where you would look for it.**

The brief asks for bulk publish from Curated to Live as an operation on the admin
content list. The kernel's content list at `/admin/content` offers exactly three
bulk actions — publish, unpublish, delete — as three string literals in a
`matches!`, re-matched below with `unreachable!()` on the other arms. There is no
tap, no registry, no alter hook and no data-driven list: **a plugin cannot add a
fourth bulk action to that screen.** `G-BULK-ACTIONS-NOT-EXTENSIBLE`.

Note also that `publish` there is not publishing in the brief's sense. It sets
`status`, the draft flag. It does not touch the stage, so it cannot make anything
public that is sitting on Incoming.

So bulk publish lives on Ritrovo's own screen, `/admin/content/editorial`,
described in Walkthrough 1. Two further consequences of it being a plugin route,
both visible:

- **No flash message, no redirect.** A plugin's response has no headers field, so
  a 302 would go out with no `Location`, and no host call can write the session
  the kernel's flash messages live in (`G-PLUGIN-ROUTE-NO-HEADERS`). The POST
  renders its own result page instead. Reloading that page re-submits the POST;
  the form carries the stage it acted on and the UPDATE is conditional on the
  source stage, so a re-submission moves nothing a second time.
- **It is not in the admin sidebar.** That navigation is hardcoded in
  `templates/page--admin.html` and keys on two specific plugin names.

### Progress display

There is none, and the brief asks for one ("progress tracking via Redis
polling"). The kernel has a batch service that creates and polls a Redis progress
record, and **nothing anywhere executes a batch**: `update_progress`, `complete`
and `fail` have no callers outside their own module, and `operation_type` is a
free string with no implementation behind any value. `G-BATCH-NO-EXECUTOR`.

The kernel's own bulk publish does not use it either: it loops over the selected
ids inline in the request. Ritrovo's editorial screen does the same, for the same
reason, and the selection is bounded by a 50-row page.

---

## Walkthrough 4: revisions

**This one does not work, and the failure is the kernel's.** It is written up
rather than demonstrated because there is nothing to demonstrate.

What the brief asks for is edit, preview the draft, revert, and compare. Here is
the state of each at the pinned release.

1. **Edit a conference.** Works. Log in as `editor_alice`, open a conference on
   Incoming, `/item/<id>/edit`, change the title, save. The kernel writes an
   `item_revision` row inside the same transaction as the update, every time,
   with no change detection.

2. **See its history.** **500.** `/item/<id>/revisions` fails for any item that
   has a revision. `ItemRevision` declares two fields, `change_summary` and
   `ai_generated`, that neither revision query selects and that carry no
   `#[sqlx(default)]`, so decoding a row fails on a missing column.

   Reproduced directly: the same conference answered **200** with zero revisions
   and **500** with one. It is every item created through the kernel, not merely
   every edited one, because `Item::create` writes an initial revision of its own.
   The imported conferences are the only ones whose history page answers, and
   only because the importer writes raw SQL and creates no revision at all.
   `G-REVISION-HISTORY-500`.

3. **Revert.** Unreachable twice over. `revert_to_revision` calls the same broken
   query as its first statement, so it 500s; and the only button that reaches it
   lives inside the loop of the history template, on the page that 500s before it
   renders. The access check runs first, so an unauthorized caller still gets a
   correct 403 and an authorized one gets a 500.

4. **Compare two revisions.** Does not exist. No route, no handler, no service
   method, no template. The `change_summary` column that was clearly meant to
   carry a diff is written by nothing and read by nothing.
   `G-REVISION-NO-COMPARE`.

5. **Preview a draft.** Does not exist either. `/item/{id}` always renders the
   current row and takes no revision parameter. The nearest thing is the session's
   active stage, which is set only by a JSON endpoint requiring a CSRF header and
   administrator rights, and which no template in the kernel calls.

**Ritrovo did not build a replacement.** It could: the revision rows are readable
through the `db` host call, and a plugin could serve its own history, comparison
and even revert. That was considered and rejected for this pull request — it
would mean re-implementing a kernel subsystem the kernel claims to own, and
hiding a defect that ought to be visible. The fix is four columns in two SELECT
lists, in the kernel.

What this costs the demo: of the brief's five revision scenarios, only
**emergency unpublish** can be shown end to end, using the Published checkbox on
the edit form. Its audit trail is real in the database and invisible in the UI,
for the same reason as everything above.

---

## What the search index contains, and why

**The Pagefind index an anonymous visitor searches holds published Live items
only.** The mechanism is a build-time filter, not per-viewer filtering: the
indexer selects `WHERE status = 1 AND stage_id = $1` bound to the Live stage, and
writes one global index to disk that every viewer is served.

Verified on the run above: the index was rebuilt as the demo filled, ending at
**83 items**, which is exactly the count of published items on Live, while 5,561
conferences on Incoming were absent from it. It tracked the seed and the manual
promotions: 21 items, then 81, then 83.

Two things follow that are worth stating plainly.

- **It is the same index for an administrator as for a visitor.** Nothing is
  filtered at query time, so "the index is Live-only" is a property of what was
  built, and an editor searching the site does not find Incoming conferences
  through it.
- **It carries no field-level access filtering.** `tap_field_access` never runs
  over the indexer, so a restricted field on a published Live item is in the
  public index if `search_field_config` lists it. `editor_notes` is not listed,
  which is what keeps it out today; that is configuration holding the line, not
  access control.

The server-side half of `/search` is a different engine — Postgres full-text,
filtered by the session's stage — and the client hides its results as soon as
`scolta.js` loads. That split, and what it does when the index is missing, is
`G-SEARCH-PAGE-BLANK-WITHOUT-INDEX`.

---

## Checking all of this without reading it

`scripts/verify-demo.sh` asserts the load-bearing claims above on a running demo:
that each of the three accounts can log in; that `/admin/content/editorial`
answers 401 to a visitor, 403 to `viewer_carol` and 200 to `editor_alice`; that a
conference on Incoming answers 404, 404 and 200 to the same three; that Publish
to Live is offered to the publisher and not to the editor; and that the importer
is still landing conferences on Incoming.

It fails on the first broken one and says which.
