# `seed-italian/` — fifteen Italian conferences, in the kernel's translation shape

A separate config import, run by `scripts/demo-bootstrap.sh` after the plugins are
enabled. Import reads the `.yml` files only, so this file is not part of it.

## Three kinds of file

- `item.<id>.yml` — the conference, **in English**, with `language: en` and
  `field_language: "it"`: the record is the site-default text of a conference that
  is *held* in Italian. Brand names are kept as they are ("Codemotion Roma 2026");
  descriptive titles are translated.
- `item_translation.<id>.it.yml` — its Italian: `item_id`, `language`, `title` and
  the translated `fields`. The kernel's own config entity; import writes it into
  the kernel's `item_translation` table, and `/it/item/<id>` overlays it. No plugin
  keeps a copy.
- `url_alias.<uuid>.yml` — two per conference: `/conferences/<english-slug>` in
  `en` and `/conferenze/<italian-slug>` in `it`, both pointing at `/item/<id>`.
  Stored without the `/it` prefix, because the kernel strips that prefix before it
  looks an alias up (see `../README.md`).

## Why the record is English and the translation Italian

Not the other way round, though these are Italian conferences. The kernel applies a
translation only when the request language is **not** the site default
(`crates/kernel/src/routes/item.rs:349` at 0.103.0,
`G-TRANSLATION-OVERLAY-SKIPS-THE-DEFAULT-LANGUAGE` in `FRICTION.md`). An item stored
in Italian with an English translation would render Italian at `/item/<id>` and
Italian again at `/it/item/<id>`: the English would never be shown. So the stored
record carries the default language, and `field_language` carries the fact the
listing filters on, which is the language the conference is held in.

## Descriptions are block lists

`field_description` is an Editor.js block list in both files, never a string:
`render_blocks` sanitizes a list and returns a string untouched
(`G-RENDER-BLOCKS-PASSES-STRINGS-UNSANITIZED`). The kernel's translate screen
edits Text and TextLong fields only, so an editor changes a seeded description in
these files or on the item, not on that screen (`G-TRANSLATE-SCREEN-TEXT-FIELDS-ONLY`).
