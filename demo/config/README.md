# The demo's own config set

Two files' worth of configuration, imported by `scripts/demo-bootstrap.sh` as a
set of its own, after Trovato's tutorial set. Everything else the demo needs
comes from `docs/tutorial/config` inside the released image; this directory holds
only what that set does not carry.

## `variable.site_front_page.yml`

The front page. The tutorial tells the reader to set it by hand at
`/admin/config/site` once the import has finished, which a one-command demo
cannot do. A `variable` config entity writes the same `site_config` row the form
writes, so importing it is the non-interactive equivalent.

## `url_alias.*.yml` — the Italian listing paths

Three aliases (`/conferenze`, `/relatori`, `/argomenti`) declared in Italian.

Trovato's tutorial set already declares these, as `/it/conferenze`,
`/it/relatori` and `/it/argomenti` with `language: it`. Those rows cannot ever
match a request, and the reason is a mismatch inside the kernel between its own
config set and its own middleware:

1. `negotiate_language` runs first. It recognises `/it/` as a language prefix,
   resolves the language to `it`, and **strips the prefix from the URI**.
2. `resolve_path_alias` then looks the remaining path up. For a request to
   `/it/conferenze` that path is `/conferenze`, and the language is `it`.
3. No row has `alias = /conferenze, language = it`, so there is no alias, and
   `/conferenze` is not a route either. The request 404s.

Observed on the released kernel, 0.101.0: `/it/conferenze` answers 404 while
`/it/conferences` answers 200 with `lang="it"`, which is the same alias lookup
succeeding on the English row.

So the aliases here are stored the way the middleware asks for them: the
prefix-less path, in Italian. They are additive — the kernel's own rows stay
where they are — and they are why the Italian URLs the tutorial advertises work
in this demo. Fixing the tutorial set's rows is a Trovato change, and Ritrovo
does not make Trovato changes.
