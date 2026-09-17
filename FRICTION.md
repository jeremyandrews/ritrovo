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
