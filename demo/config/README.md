# Ritrovo's configuration set

The content model and everything that hangs off it: the `conference` and
`speaker` types and their fields, the `topics` category and its 44 terms, the
gathers, the roles, the editorial stages, the workflow variable, the search field
configs, the tiles, the menu links, the aliases, the pathauto patterns, the
languages, the locale file and the seeded content.

`scripts/demo-bootstrap.sh` imports this directory, before it enables any plugin,
and the host-in-the-loop suites import it too. It is the set, not a copy of one.

## It has to be imported as a whole

Config import resolves every reference across the directory first and then the
database, and refuses the entire set if one reference does not resolve, writing
nothing. The search field configs name the `conference` and `speaker` types, the
tiles and aliases name the gathers, every tag names its category and its parents,
and the seed's items name the type and the stage. So there is no useful subset,
and nothing here can be imported on its own.

Subdirectories are NOT part of it. Import reads the `.yml` files in the directory
it is given and skips directories, which is why `seed-content/` and
`seed-italian/` are separate imports, after the plugins are enabled.

## It used to be Trovato's

Until 2026-09-18 this directory held two files the kernel image's
`docs/tutorial/config` did not carry, and the model itself lived in that image.
The model is Ritrovo's to define, so the whole set moved here.

The kernel still ships its copy and its tutorial still imports it, so the two
diverge, which is the point. `scripts/check-tutorial-templates.sh` reports the
difference and does not fail on it. `tests/host/tutorial-config/`, a subset the
suites used to import, is gone.

## `variable.site_front_page.yml`

The front page. The tutorial tells the reader to set it by hand at
`/admin/config/site` once the import has finished, which a one-command demo
cannot do. A `variable` config entity writes the same `site_config` row the form
writes, so importing it is the non-interactive equivalent.

## `menu_link.0193a5a0-0004-7000-8000-000000000003.yml` — "Call for Papers"

Points at `/cfps`. The kernel's copy of this row points at `/open-cfps`, which no
route and no alias answers: the gather it means is `ritrovo.open_cfps`, whose
`canonical_url` and `url_alias` are both `/cfps`. The link 404ed from the site's
own navigation on a stock import. Corrected here while the row was still an
override, and kept when the set moved. `G-TUTORIAL-CONFIG-SET-DEFECTS`.

## `url_alias.*.yml` — the Italian listing paths

Three aliases (`/conferenze`, `/relatori`, `/argomenti`) declared in Italian.

This set ALSO declares them the other way, as `/it/conferenze`, `/it/relatori`
and `/it/argomenti` with `language: it`, because those rows came with the set when
it moved and correcting them is A8's row, not this one. Those rows cannot ever
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

So three more aliases are stored the way the middleware asks for them: the
prefix-less path, in Italian. They are additive, and they are why the Italian URLs
work in this demo at all. Both sets of rows are now Ritrovo's, so the doubled ones
could simply be deleted; that is a change to what the Italian URLs do, which is
story 38.2 and belongs to A8.
