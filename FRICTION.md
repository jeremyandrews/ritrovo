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

### G-VIEW-TAP-INPUT-CARRIES-NO-VIEWER: **[Low, NEW]** the view tap's input is the Item alone, and the documented signature that would carry the viewer does not exist

`ItemService::load_for_view` serializes the `Item` and dispatches `tap_item_view`
with it (`crates/kernel/src/content/item_service.rs:576-583`); the SDK tap takes an
`Item` and returns a `String`. The plugin documentation describes a different tap,
`tap_item_view(input: ItemViewInput) -> RenderElement`
(`docs/plugin-development.md:215,292`, `docs/plugin-quick-reference.md:54,72`), and
there is no `ItemViewInput` type in the SDK.

`ritrovo_access::tap_item_view` has been an empty placeholder since it was written,
on the stated grounds that "the kernel does not pass `UserContext` to view taps".
That is half right, and the half that is wrong matters more:

- The viewer is reachable. The tap is dispatched with the viewer's request state
  (`item_service.rs:577`), so `current-user-id` and `current-user-has-permission`
  (`crates/kernel/src/host/user.rs:14-54`) answer for the person viewing the page.
  They are host calls rather than input, and the permission call has no
  `administer site` bypass (`G-USER-API-NO-ADMIN-BYPASS`, below).
- The job the placeholder was waiting to do is not a view-tap job. A view tap's
  output is HTML appended to the page (`item_service.rs:585-590`); it cannot take a
  field out. Hiding `field_editor_notes` from non-editors is what `tap_field_access`
  is for, and the kernel applies it before any view tap runs
  (`item_service.rs:573`, dispatched at `:1236`, with the viewer in
  `FieldAccessBatchInput`).

**Impact for Ritrovo.** None in the kernel to wait for: stripping editor notes is
a `tap_field_access` implementation in `ritrovo_access`, not a kernel change. The
placeholder stays as it is for now, with a comment that says so. The friction is
the documentation, which describes a view tap with the viewer in its input and a
structured return that a plugin author would reasonably wait for.

**Recommendation.** Correct the two references to the real signature
(`Item -> String`, output appended, `G-VIEW-OUTPUT-JSON-ENCODED` decoding), and say
in both where a plugin gets the viewer (the user host calls) and where field
visibility belongs (`tap_field_access`).

### G-MAIL-HOST-CANNOT-REACH-A-USER: **[Medium, NEW]** a plugin has no way to send mail to one of the site's own users

The mail host has one function, `send-to-site-contacts`, and the recipient is not a
parameter: it is always the site's configured contact address
(`crates/kernel/src/host/mail.rs:1-20`, registered at `:91-98`). The module docs
argue for that, and the argument is right about the thing it is guarding against:
a host function that mails whatever address a plugin names is a relay.

It also rules out every message a site sends to its own members on a plugin's
behalf: a subscription notice, a digest, a reply to something they posted.

**Impact for Ritrovo.** `ritrovo_notify`'s design brief sends notification and
digest email to subscribers (`docs/ritrovo/overview.md`, "`ritrovo_notify`",
"Cron"). No plugin can do that on this kernel. When the plugin is built (A7) its
deliverable is on-site notifications and a logged digest, which is the brief's own
fallback for the demo, until a surface exists. `plugins/ritrovo_notify/README.md`
says so.

**Recommendation.** A `send-to-user(user_id, subject, body)` in which the kernel,
not the plugin, resolves the address from `users.mail`, refuses users who are
blocked or have not verified their address, and honours a per-user opt-out. The
recipient is then a person who has an account on this site and has not said no,
which is not a relay, and the plugin never sees an address.

### G-USER-API-NO-ADMIN-BYPASS: **[Medium, RESIDUAL]** `current-user-has-permission` is a literal membership test, so an administrator fails a plugin's permission check

Recorded by netgrasp-trovato; re-confirmed at this revision from this consumer's
side. Every kernel route treats `administer site` as a superuser
(`UserContext::is_admin`, and for example the plugin API gate at
`crates/kernel/src/routes/plugin_api.rs:277`). The host function a plugin uses to
make the same decision checks the literal string and nothing else
(`crates/kernel/src/host/user.rs:49`).

**Impact for Ritrovo.** Any view-time decision `ritrovo_access` or `ritrovo_notify`
makes with it ("is this viewer an editor", "may this viewer subscribe") would treat
a site administrator without the literal permission as a stranger, while the
kernel's own routes wave the same person through. Nothing in Ritrovo calls it
today; both of those plugins will.

**Recommendation.** As recorded there: give the host call the same `is_admin()`
short-circuit the routes use, or expose `current-user-is-admin` so a plugin can
apply it deliberately.

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
