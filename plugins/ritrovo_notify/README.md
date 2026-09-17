# ritrovo_notify

**Status: not built.** The plugin installs, enables, creates its
`pending_notifications` table and does nothing else. It is built for real later
in this series (A7). Until then it declares no taps, because every one it used to
declare had nothing behind it: a `/user/subscriptions` menu entry with no
`tap_api`, so the path 404ed; a Subscribe button on every conference page with no
form or endpoint, so it did nothing; a `ritrovo_notifications` queue with no
worker; and two permissions nothing checked.

This file is what it is meant to become, taken from the design brief in
[`docs/ritrovo/overview.md`](../../docs/ritrovo/overview.md) ("`ritrovo_notify`",
"Plugin-to-Plugin", "Cron", "Authenticated" role, the subscribe endpoints and the
"My Subscriptions" tile). Where the pinned kernel (Trovato 0.102) constrains a
piece of it, that is said next to the piece.

## What it does

A signed-in visitor subscribes to a conference and hears about it when something
that matters changes: its dates, its venue, or its call for papers closing. They
choose whether that arrives as it happens or as a daily digest.

## Taps

| tap | behaviour |
|---|---|
| `tap_perm` | `manage own subscriptions` (subscribe, unsubscribe, see your own list) and `administer notifications` (settings and the queue). Declared again only when something checks them. |
| `tap_item_view` | On a conference page, for an authenticated viewer holding `manage own subscriptions`, a Subscribe or Unsubscribe toggle reflecting that viewer's current state. Nothing for anonymous visitors. The toggle submits to the endpoints below, as a plain form first and progressively enhanced, so it works without JavaScript. |
| `tap_menu` + `tap_api` | `/user/{uid}/subscriptions`, the visitor's own list, private to them, with an unsubscribe control per row; and `POST` / `DELETE /api/v1/conferences/{id}/subscribe`, authenticated. Every menu entry is a `MenuRoute::api` with its callback served by `tap_api`, so no path is registered without a handler. |
| `tap_item_update` | When a conference someone is subscribed to changes its dates, venue or CFP, one notification per subscriber goes onto the queue. |
| `tap_queue_info` + `tap_queue_worker` | Declares `ritrovo_notifications`, and drains it: an immediate notification is sent, a digest one is written to `pending_notifications` for cron. |
| `tap_cron` | Once a day, each user's unsent `pending_notifications` rows become one digest, and are marked sent. |

## Plugin to plugin

`ritrovo_cfp` writes a `cfp_closing_soon` event to the `ritrovo_notifications`
queue when a conference's CFP enters its last seven days, and this plugin's worker
turns it into notifications for that conference's subscribers. It is the
repository's example of two plugins cooperating through shared queue
infrastructure rather than calling each other. `ritrovo_cfp` does not write the
event today either; the two halves land together.

## State

- **Subscriptions.** The kernel already keeps subscriptions in a
  `user_subscriptions` table (`crates/kernel/src/models/subscription.rs` at the
  pinned revision). Reuse it rather than adding a second one. A plugin reaches it
  only through the `db` host with the table declared in its manifest; whether the
  kernel's table policy admits a kernel-owned table there is the first thing to
  check when this is built.
- **Pending notifications.** `migrations/001_create_pending_notifications.sql`,
  already applied wherever this plugin has been enabled:
  `user_id`, `event_type`, `item_id`, `payload`, `created`, `sent`, `sent_at`.
- **Preferences.** Digest frequency per user, set on the profile form's
  notification preferences.

## Constraints the kernel puts on it

- **Email to a subscriber is not reachable from a plugin.** The kernel's mail
  host function sends only to the site's own configured contact address, by
  design (`crates/kernel/src/host/mail.rs`), so a plugin cannot mail a user. The
  design brief's emailed digest therefore needs a kernel surface that does not
  exist yet; see `FRICTION.md`, `G-MAIL-CANNOT-REACH-A-USER`. Until then the
  deliverable is the on-site list and digest, and the brief's own fallback for the
  demo, which is to log instead of send.
- **No scheduler.** `tap_cron` runs when something calls the kernel's cron route,
  as the importer's does; the demo's cron poker already does.
- **Checking the viewer's permission in a view tap** goes through
  `current-user-has-permission`, which does not treat `administer site` as a
  superuser (`FRICTION.md`, `G-USER-API-NO-ADMIN-BYPASS`).

## Done means

A host-in-the-loop suite in `tests/ritrovo_notify_host.rs` that subscribes a user
through the real route, changes the conference through `ItemService`, drains the
queue with the kernel's drain, runs the digest from `tap_cron`, and asserts the
rows and the rendered toggle for a subscriber, a non-subscriber and an anonymous
visitor. The test that currently asserts this plugin registers nothing is deleted
in the same change.
