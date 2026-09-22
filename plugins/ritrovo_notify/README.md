# ritrovo_notify

**Status: partly built.** A signed-in member follows a conference and sees what
they follow on a page of their own. Three of the seven taps below are real on
Trovato 0.103.0; the other four are blocked on the kernel, and each says what it
waits on.

This plugin was a placeholder until 0.103.0 because of one gap. Every tap it
used to declare had nothing behind it — a `/user/subscriptions` menu entry with
no `tap_api` so the path 404ed, a Subscribe button with no endpoint so it did
nothing, a queue with no worker, and two permissions nothing checked — and the
gap that kept the rest unbuildable was `tap_perm`, which was declared in the WIT
and dispatched nowhere, so no role could hold a plugin's permission. 0.103.0
dispatches it at boot into a registry and a `plugin_permission` table that
config import reads, and `demo/config` grants the permission this plugin
declares. `FRICTION.md`, `G-PERM-TAP-NOT-DISPATCHED`.

## What it does

A signed-in member subscribes to a conference and hears about it when something
that matters changes: its dates, its venue, or its call for papers closing. The
subscribing half is built. The hearing-about-it half is not, and cannot be: see
"What is not built".

## Taps

| tap | state | behaviour |
|---|---|---|
| `tap_perm` | **built** | `manage own subscriptions`, gating all three routes below. Granted to the `authenticated user` role in `demo/config`, and the kernel checks it before dispatch. |
| `tap_menu` + `tap_api` | **built** | `GET /user/{uid}/subscriptions`, the member's own list, private to them; `POST .../subscribe` and `POST .../unsubscribe`, as `_token` form posts. Every menu entry is a `MenuRoute::api` whose callback `tap_api` serves. |
| `tap_queue_info` | **declared, no worker** | Declares `ritrovo_notifications` as the JSON array the kernel reads, with `concurrency` and no key the kernel ignores. The one deliberate exception to the rule below; the argument is in `src/lib.rs`. |
| `tap_item_view` | **not built, and would be invisible** | The Subscribe toggle on the conference page. Two gaps compose to close this; see "Why the toggle is not on the conference page". |
| `tap_item_update` | **blocked** | Queue a notification when a subscribed conference changes. A plugin's item write fires no taps, so the importer changing a conference notifies nothing. `G-ITEM-API-BYPASSES-ITEM-SERVICE`. |
| `tap_queue_worker` | **blocked** | Nothing can fill the queue (below), and the delivery path is the mail gap. |
| `tap_cron` | **blocked** | The daily digest is email, and a plugin cannot send mail to a site's own member. `G-MAIL-CANNOT-REACH-A-USER`, `G-MAIL-UNAVAILABLE-IN-BACKGROUND`. |

**The rule every tap here follows: nothing is declared that has nothing behind
it.** That is what this plugin was emptied for. It is why `administer
notifications`, which the earlier version of this file listed, is not declared:
it would gate a settings screen and a queue view, neither of which exists, so it
would be a permission an administrator could grant to no effect. It lands with
the screen it is for.

## Where a subscription is stored

**In the kernel's `user_subscriptions` table**, not a private one. The table is
`(user_id, item_id, created)` with the pair as its primary key and cascade
deletes to both `users` and `item`, created by a kernel migration and called by
nothing in the kernel.

The kernel's table was tried first, as it should be: a subscription its own
`Subscription` model can read is worth more than one only this plugin knows
about. The database policy admits it. `crates/kernel/src/plugin/db_policy.rs`
derives a plugin's allowlist as the union of the tables its own migrations
create and the tables its manifest names in `db_tables`, and checks a table by
set membership — there is no ownership concept and no denylist of kernel tables.
Two plugins shipped in the kernel image already rely on that: `trovato_book`
declares `item`, `trovato_spam` declares `comment`. So the fallback of a private
table and a migration of this plugin's own was not needed.

The honest footnote is that `db_tables` is advisory here. The SDK wraps only
`query_raw` and `execute_raw`, not the four structured `db` calls the WIT
declares, so every statement is raw SQL — and the raw-SQL gate reads only the
`raw_sql` flag and never consults the table list. The declaration is still the
right one to make, because it is the one an auditor reads.

## Why the toggle is not on the conference page

It cannot be, on this kernel, and it takes two gaps together to make that true.

- **The item template's context carries no viewer.** The kernel builds a fresh
  context with seven keys — `item`, `children`, `referenced_items`,
  `reverse_references`, `safe_urls`, `active_language`, `text_direction` — and
  renders `elements/item--conference.html` from it. The viewer is loaded for
  that request and used for access control, and never put in the context. So a
  template cannot tell a signed-in visitor from an anonymous one, and a
  Subscribe control rendered there would be shown to everyone including the
  visitors it would refuse. `G-ITEM-TEMPLATE-HAS-NO-VIEWER`.
- **A view tap knows the viewer and cannot be seen.** `tap_item_view` runs with
  the viewer's request state, so `current-user-id` answers for the person
  viewing. But its output is appended into `children`, the same string the
  kernel fills with a generic dump of every scalar field, and this repository's
  conference template does not render `children` for exactly that reason. There
  is no handle on the plugin half alone.
  `G-RENDER-CHILDREN-MIXES-FIELD-DUMP-AND-PLUGIN-OUTPUT`.

So the control lives on this plugin's own page, which is authenticated, and
every response states the resulting state in words. The member reaches it from
the user menu: the entry is gated on a permission no anonymous visitor holds,
and the kernel filters the menu it renders to what the viewer may open. **An
anonymous visitor is never shown a control that would refuse them**, which is
the rule this design exists to keep.

`/user/{uid}/subscriptions?conference={uuid}` renders the toggle for one named
conference. That is the address a link on a conference page would point at, the
day the kernel separates plugin output from the field dump.

There is no AJAX half and no JavaScript at all. The kernel's form AJAX is
administrator-only and routes through a form service no route calls
(`G-AJAX-ADMIN-ONLY-NO-CONDITIONAL-FIELDS`).

## What is not built, and why

Each of these keeps a blocked row in `docs/ritrovo/STATUS.md`. No workaround is
built for any of them.

- **Daily digest email** and **comment and change notifications to
  subscribers.** A plugin cannot send mail to one of the site's own users: the
  mail host has one function and the recipient is always the site's configured
  contact address. Mail also fails from cron and queue workers, which is where
  every message here would be sent from, because background taps are built with
  no email service at all. `G-MAIL-CANNOT-REACH-A-USER`,
  `G-MAIL-UNAVAILABLE-IN-BACKGROUND`. Re-derived at 0.103.0: both still stand.
- **`ritrovo_cfp` emitting `cfp_closing_soon` onto this plugin's queue.** The
  queue host stamps the calling plugin's own name onto every job and the drain
  hands a job back to that same plugin, so one plugin cannot enqueue onto
  another's. `G-QUEUE-NO-CROSS-PLUGIN`.
- **`tap_queue_worker`.** Nothing can fill the queue, and its delivery path is
  the mail gap.
- **A shared table both plugins own by contract**, as a way round the queue gap.
  Deliberately not built. The gap is on the kernel's own backlog as a Ritrovo
  gate, so the fix is coming; a convention-only table built now is thrown away
  when it lands, after teaching two plugins to depend on it.
- **Mailpit in the demo compose.** An SMTP sink with nothing able to send
  through it.

## Pending notifications

`migrations/001_create_pending_notifications.sql` creates the table the digest
would write: `user_id`, `event_type`, `item_id`, `payload`, `created`, `sent`,
`sent_at`. It is applied wherever this plugin has been enabled and nothing
writes it yet, because its producer and its consumer are both blocked above. It
is kept rather than dropped because the table is not the blocked part.

## Done means

`tests/ritrovo_notify_host.rs`, against the compiled module on the real kernel
with a real Postgres and a real Redis session: a member subscribes through the
real route and the row lands in `user_subscriptions`; unsubscribes and it is
gone; sees their own list and is refused another member's; an anonymous visitor
is served no control and a direct post writes nothing; a member posts a comment
and a threaded reply nests under it; the queue declaration loads and the kernel
reads it as an array; and the declared permission reaches `plugin_permission`.

The rendered half — the comment form a signed-in member sees, the depth class on
a reply, the moderation queue an editor opens — is checked by
`scripts/verify-demo.sh` against the released image, because the host suites
configure no `TEMPLATES_DIR` and the kernel's templates are not on disk there.
