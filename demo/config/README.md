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

## The footer, and the pages behind it

`menu_link.…05` to `…09` are the brief's footer: About, Contact, API
documentation, Data sources and Privacy. Every one points at a kernel `page`
item: four are in `seed-content/` with an alias each, and the API documentation
page is written by `scripts/demo-bootstrap.sh` from `docs/API.md` on every start,
so it cannot drift from the reference. `verify-demo.sh` follows every link the
main and footer menus render.

## `search_field_config.*.yml` — the brief's weights

Title A, description B, city C, topics C (and country C, which the brief does
not list). Two of those read values `ritrovo_importer` writes for search on
every cron call, because the kernel cannot derive them: `title` (the Pagefind
export reads only `fields`, `G-PAGEFIND-INDEX-OMITS-TITLES`) and
`field_topic_names` (tag uuids are not words,
`G-SEARCH-CANNOT-INDEX-CATEGORY-LABELS`).

## `url_alias.*.yml` — the Italian listing paths

Three aliases (`/conferenze`, `/relatori`, `/argomenti`) declared in Italian,
stored **without** the `/it` prefix. That is the only form a request can match:

1. `negotiate_language` runs first. It recognises `/it/` as a language prefix,
   resolves the language to `it`, and **strips the prefix from the URI**.
2. `resolve_path_alias` then looks the remaining path up, `/conferenze`, in `it`
   and then in `en` (`G-ALIAS-FALLBACK-HARDCODES-EN` in `FRICTION.md`).

The set used to carry the same three a second time as `/it/conferenze`,
`/it/relatori` and `/it/argomenti`, which no request could ever reach, since the
middleware strips that prefix before the lookup. A8 deleted them (story 38.2).
What each address renders, re-checked on 0.103.0:

| Address | `html lang` | |
|---|---|---|
| `/it/conferenze` | `it` | the upcoming gather, Italian page |
| `/it/relatori` | `it` | the speakers gather, Italian page |
| `/it/argomenti` | `en` | the topics page ignores the request language (`G-TOPICS-PAGE-IGNORES-LANGUAGE`) |
| `/it/item/{id}` of a translated conference | `it` | the kernel's translation overlay |

The seed's own conferences carry two aliases each, `/conferences/<slug>` in
English and `/conferenze/<slug>` in Italian; see `seed-italian/README.md`.
